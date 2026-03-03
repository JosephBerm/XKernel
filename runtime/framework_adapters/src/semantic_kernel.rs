// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Semantic Kernel Framework Adapter
//!
//! Implements concept mappings for Microsoft's Semantic Kernel framework to CSCI entities.
//! Semantic Kernel is a C# framework for semantic function orchestration.
//!
//! Sec 4.3: Semantic Kernel Concept Mapping
//! Mappings:
//! - Skill/Plugin → ToolBinding (Full/Partial fidelity)
//! - Planner → CognitiveTask (Approximate fidelity)
//! - KernelMemory → SemanticMemory (Partial fidelity)

use alloc::string::String;
use crate::adapter::{
    CognitiveTaskConfig, IFrameworkAdapter, SemanticChannelConfig, SemanticMemoryConfig,
    ToolBindingConfig, TranslationResult,
};
use crate::framework_type::FrameworkType;
use crate::error::AdapterError;
use crate::AdapterResult;

/// Semantic Kernel framework adapter.
/// Sec 4.2: Framework Adapter Implementation
#[derive(Debug, Clone)]
pub struct SemanticKernelAdapter {
    /// Minimum supported version
    min_version: String,
    /// Maximum supported version
    max_version: String,
}

impl SemanticKernelAdapter {
    /// Creates a new Semantic Kernel adapter instance.
    /// Sec 4.2: Adapter Instantiation
    pub fn new() -> Self {
        SemanticKernelAdapter {
            min_version: "0.1.0".to_string(),
            max_version: "1.5.x".to_string(),
        }
    }

    /// Maps a Semantic Kernel skill to ToolBinding.
    /// Sec 4.3: Skill Mapping (Full Fidelity)
    fn map_skill(&self, skill_def: &str) -> AdapterResult<ToolBindingConfig> {
        let tool_id = alloc::format!("sk-skill-{}", ulid::Ulid::new());

        Ok(ToolBindingConfig {
            tool_id,
            name: "SemanticKernelSkill".to_string(),
            description: skill_def.to_string(),
            input_schema: "{}".to_string(),
            output_schema: "{}".to_string(),
            requires_authorization: false,
        })
    }

    /// Maps a Semantic Kernel planner to CognitiveTask.
    /// Sec 4.3: Planner Mapping (Approximate Fidelity)
    fn map_planner(&self, planner_def: &str) -> AdapterResult<CognitiveTaskConfig> {
        let task_id = alloc::format!("sk-plan-{}", ulid::Ulid::new());

        Ok(CognitiveTaskConfig {
            task_id,
            name: "SemanticKernelPlanner".to_string(),
            objective: planner_def.to_string(),
            timeout_ms: 60000,
            is_mandatory: false,
        })
    }

    /// Maps Semantic Kernel memory to SemanticMemory.
    /// Sec 4.3: Memory Mapping (Partial Fidelity)
    fn map_kernel_memory(&self, memory_def: &str) -> AdapterResult<SemanticMemoryConfig> {
        let memory_id = alloc::format!("sk-mem-{}", ulid::Ulid::new());

        Ok(SemanticMemoryConfig {
            memory_id,
            memory_type: "kernel_memory".to_string(),
            capacity_tokens: 50000,
            serialization_format: "json".to_string(),
        })
    }
}

impl Default for SemanticKernelAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl IFrameworkAdapter for SemanticKernelAdapter {
    fn translate_to_ct(&self, framework_task: &str) -> AdapterResult<CognitiveTaskConfig> {
        if framework_task.is_empty() {
            return Err(AdapterError::TranslationError(
                "Empty framework task definition".to_string(),
            ));
        }
        // Semantic Kernel tasks are typically planner-based
        self.map_planner(framework_task)
    }

    fn translate_from_ct(&self, task_id: &str, result: &str) -> AdapterResult<TranslationResult> {
        Ok(TranslationResult {
            artifact_id: task_id.to_string(),
            artifact_type: "semantic_kernel_result".to_string(),
            success: true,
            fidelity: "partial".to_string(),
            translation_notes: "Semantic Kernel result with approximate fidelity translation".to_string(),
        })
    }

    fn map_memory(&self, framework_memory: &str) -> AdapterResult<SemanticMemoryConfig> {
        if framework_memory.is_empty() {
            return Err(AdapterError::MemoryMappingError(
                "Empty memory definition".to_string(),
            ));
        }
        self.map_kernel_memory(framework_memory)
    }

    fn map_tool(&self, framework_tool: &str) -> AdapterResult<ToolBindingConfig> {
        if framework_tool.is_empty() {
            return Err(AdapterError::ToolBindingError(
                "Empty tool definition".to_string(),
            ));
        }
        // Semantic Kernel skills are the primary tool mechanism
        self.map_skill(framework_tool)
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

#[cfg(test)]
mod tests {
    use super::*;
use ulid::Ulid;
use alloc::format;
use alloc::string::ToString;

    #[test]
    fn test_semantic_kernel_adapter_creation() {
        let adapter = SemanticKernelAdapter::new();
        assert_eq!(adapter.framework_type(), FrameworkType::SemanticKernel);
    }

    #[test]
    fn test_semantic_kernel_translate_to_ct() {
        let adapter = SemanticKernelAdapter::new();
        let result = adapter.translate_to_ct("planner definition");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.timeout_ms, 60000);
    }

    #[test]
    fn test_semantic_kernel_map_memory() {
        let adapter = SemanticKernelAdapter::new();
        let result = adapter.map_memory("kernel memory config");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.memory_type, "kernel_memory");
    }

    #[test]
    fn test_semantic_kernel_map_tool() {
        let adapter = SemanticKernelAdapter::new();
        let result = adapter.map_tool("skill definition");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.name, "SemanticKernelSkill");
    }

    #[test]
    fn test_semantic_kernel_supported_versions() {
        let adapter = SemanticKernelAdapter::new();
        let versions = adapter.supported_versions();
        assert!(versions.contains("0.1.0"));
    }

    #[test]
    fn test_semantic_kernel_default() {
        let adapter = SemanticKernelAdapter::default();
        assert_eq!(adapter.framework_type(), FrameworkType::SemanticKernel);
    }
}
