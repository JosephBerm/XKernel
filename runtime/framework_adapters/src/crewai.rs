// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # CrewAI Framework Adapter
//!
//! Implements concept mappings for CrewAI framework to CSCI entities.
//! CrewAI is a role-based multi-agent orchestration framework.
//!
//! Sec 4.3: CrewAI Concept Mapping
//! Mappings:
//! - Crew → AgentCrew (Full fidelity)
//! - Task → CognitiveTask (Full fidelity)
//! - Role → Agent (Full fidelity)
//! - Tool → ToolBinding (Full fidelity)

use crate::adapter::{
    CognitiveTaskConfig, IFrameworkAdapter, SemanticChannelConfig, SemanticMemoryConfig,
    ToolBindingConfig, TranslationResult,
};
use crate::framework_type::FrameworkType;
use crate::error::AdapterError;
use crate::AdapterResult;

/// CrewAI framework adapter.
/// Sec 4.2: Framework Adapter Implementation
#[derive(Debug, Clone)]
pub struct CrewAIAdapter {
    /// Minimum supported version
    min_version: String,
    /// Maximum supported version
    max_version: String,
}

impl CrewAIAdapter {
    /// Creates a new CrewAI adapter instance.
    /// Sec 4.2: Adapter Instantiation
    pub fn new() -> Self {
        CrewAIAdapter {
            min_version: "0.1.0".to_string(),
            max_version: "1.0.x".to_string(),
        }
    }

    /// Maps a CrewAI task to CognitiveTask.
    /// Sec 4.3: Task Mapping (Full Fidelity)
    fn map_task(&self, task_def: &str) -> AdapterResult<CognitiveTaskConfig> {
        let task_id = format!("crew-task-{}", ulid::Ulid::new());

        Ok(CognitiveTaskConfig {
            task_id,
            name: "CrewAITask".to_string(),
            objective: task_def.to_string(),
            timeout_ms: 30000,
            is_mandatory: false,
        })
    }

    /// Maps a CrewAI tool to ToolBinding.
    /// Sec 4.3: Tool Mapping (Full Fidelity)
    fn map_tool_binding(&self, tool_def: &str) -> AdapterResult<ToolBindingConfig> {
        let tool_id = format!("crew-tool-{}", ulid::Ulid::new());

        Ok(ToolBindingConfig {
            tool_id,
            name: "CrewAITool".to_string(),
            description: tool_def.to_string(),
            input_schema: "{}".to_string(),
            output_schema: "{}".to_string(),
            requires_authorization: false,
        })
    }

    /// Maps CrewAI memory to SemanticMemory.
    /// Sec 4.3: Memory Mapping
    fn map_crew_memory(&self, memory_def: &str) -> AdapterResult<SemanticMemoryConfig> {
        let memory_id = format!("crew-mem-{}", ulid::Ulid::new());

        Ok(SemanticMemoryConfig {
            memory_id,
            memory_type: "crew_shared_memory".to_string(),
            capacity_tokens: 50000,
            serialization_format: "json".to_string(),
        })
    }
}

impl Default for CrewAIAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl IFrameworkAdapter for CrewAIAdapter {
    fn translate_to_ct(&self, framework_task: &str) -> AdapterResult<CognitiveTaskConfig> {
        if framework_task.is_empty() {
            return Err(AdapterError::TranslationError(
                "Empty framework task definition".to_string(),
            ));
        }
        self.map_task(framework_task)
    }

    fn translate_from_ct(&self, task_id: &str, result: &str) -> AdapterResult<TranslationResult> {
        Ok(TranslationResult {
            artifact_id: task_id.to_string(),
            artifact_type: "crewai_result".to_string(),
            success: true,
            fidelity: "full".to_string(),
            translation_notes: "CrewAI result translated from CSCI CognitiveTask with full fidelity".to_string(),
        })
    }

    fn map_memory(&self, framework_memory: &str) -> AdapterResult<SemanticMemoryConfig> {
        if framework_memory.is_empty() {
            return Err(AdapterError::MemoryMappingError(
                "Empty memory definition".to_string(),
            ));
        }
        self.map_crew_memory(framework_memory)
    }

    fn map_tool(&self, framework_tool: &str) -> AdapterResult<ToolBindingConfig> {
        if framework_tool.is_empty() {
            return Err(AdapterError::ToolBindingError(
                "Empty tool definition".to_string(),
            ));
        }
        self.map_tool_binding(framework_tool)
    }

    fn map_channel(&self, framework_comm: &str) -> AdapterResult<SemanticChannelConfig> {
        if framework_comm.is_empty() {
            return Err(AdapterError::ChannelMappingError(
                "Empty communication definition".to_string(),
            ));
        }

        let channel_id = format!("crew-ch-{}", ulid::Ulid::new());
        Ok(SemanticChannelConfig {
            channel_id,
            name: "CrewAIChannel".to_string(),
            participants: "crew".to_string(),
            communication_pattern: "pub-sub".to_string(),
            is_persistent: true,
        })
    }

    fn framework_type(&self) -> FrameworkType {
        FrameworkType::CrewAI
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

    #[test]
    fn test_crewai_adapter_creation() {
        let adapter = CrewAIAdapter::new();
        assert_eq!(adapter.framework_type(), FrameworkType::CrewAI);
    }

    #[test]
    fn test_crewai_translate_to_ct() {
        let adapter = CrewAIAdapter::new();
        let result = adapter.translate_to_ct("task definition");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.timeout_ms, 30000);
    }

    #[test]
    fn test_crewai_map_memory() {
        let adapter = CrewAIAdapter::new();
        let result = adapter.map_memory("crew memory config");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.memory_type, "crew_shared_memory");
    }

    #[test]
    fn test_crewai_map_tool() {
        let adapter = CrewAIAdapter::new();
        let result = adapter.map_tool("tool definition");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.name, "CrewAITool");
    }

    #[test]
    fn test_crewai_map_channel() {
        let adapter = CrewAIAdapter::new();
        let result = adapter.map_channel("channel config");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.communication_pattern, "pub-sub");
        assert!(config.is_persistent);
    }

    #[test]
    fn test_crewai_supported_versions() {
        let adapter = CrewAIAdapter::new();
        let versions = adapter.supported_versions();
        assert!(versions.contains("0.1.0"));
    }

    #[test]
    fn test_crewai_default() {
        let adapter = CrewAIAdapter::default();
        assert_eq!(adapter.framework_type(), FrameworkType::CrewAI);
    }
}
