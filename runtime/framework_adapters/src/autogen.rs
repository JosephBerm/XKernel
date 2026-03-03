// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # AutoGen Framework Adapter
//!
//! Implements concept mappings for Microsoft's AutoGen framework to CSCI entities.
//! AutoGen is a conversational AI framework for multi-agent interactions.
//!
//! Sec 4.3: AutoGen Concept Mapping
//! Mappings:
//! - Agent → Agent (Full fidelity)
//! - Function → ToolBinding (Full fidelity)
//! - Conversation → SemanticChannel (Partial fidelity)

use crate::adapter::{
    CognitiveTaskConfig, IFrameworkAdapter, SemanticChannelConfig, SemanticMemoryConfig,
    ToolBindingConfig, TranslationResult,
};
use crate::framework_type::FrameworkType;
use crate::error::AdapterError;
use crate::AdapterResult;

/// AutoGen framework adapter.
/// Sec 4.2: Framework Adapter Implementation
#[derive(Debug, Clone)]
pub struct AutoGenAdapter {
    /// Minimum supported version
    min_version: String,
    /// Maximum supported version
    max_version: String,
}

impl AutoGenAdapter {
    /// Creates a new AutoGen adapter instance.
    /// Sec 4.2: Adapter Instantiation
    pub fn new() -> Self {
        AutoGenAdapter {
            min_version: "0.2.0".to_string(),
            max_version: "0.5.x".to_string(),
        }
    }

    /// Maps AutoGen function to ToolBinding.
    /// Sec 4.3: Function Mapping (Full Fidelity)
    fn map_function(&self, function_def: &str) -> AdapterResult<ToolBindingConfig> {
        let tool_id = format!("autogen-func-{}", ulid::Ulid::new());

        Ok(ToolBindingConfig {
            tool_id,
            name: "AutoGenFunction".to_string(),
            description: function_def.to_string(),
            input_schema: "{}".to_string(),
            output_schema: "{}".to_string(),
            requires_authorization: false,
        })
    }

    /// Maps AutoGen conversation to SemanticChannel.
    /// Sec 4.3: Conversation Mapping (Partial Fidelity)
    fn map_conversation(&self, conv_def: &str) -> AdapterResult<SemanticChannelConfig> {
        let channel_id = format!("autogen-conv-{}", ulid::Ulid::new());

        Ok(SemanticChannelConfig {
            channel_id,
            name: "AutoGenConversation".to_string(),
            participants: conv_def.to_string(),
            communication_pattern: "turn-taking".to_string(),
            is_persistent: true,
        })
    }

    /// Maps AutoGen agent to cognitive task.
    /// Sec 4.3: Agent Task Mapping
    fn map_agent_task(&self, agent_def: &str) -> AdapterResult<CognitiveTaskConfig> {
        let task_id = format!("autogen-agent-{}", ulid::Ulid::new());

        Ok(CognitiveTaskConfig {
            task_id,
            name: "AutoGenAgent".to_string(),
            objective: agent_def.to_string(),
            timeout_ms: 120000,
            is_mandatory: false,
        })
    }
}

impl Default for AutoGenAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl IFrameworkAdapter for AutoGenAdapter {
    fn translate_to_ct(&self, framework_task: &str) -> AdapterResult<CognitiveTaskConfig> {
        if framework_task.is_empty() {
            return Err(AdapterError::TranslationError(
                "Empty framework task definition".to_string(),
            ));
        }
        self.map_agent_task(framework_task)
    }

    fn translate_from_ct(&self, task_id: &str, result: &str) -> AdapterResult<TranslationResult> {
        Ok(TranslationResult {
            artifact_id: task_id.to_string(),
            artifact_type: "autogen_result".to_string(),
            success: true,
            fidelity: "partial".to_string(),
            translation_notes: "AutoGen result with partial fidelity due to conversation semantics translation".to_string(),
        })
    }

    fn map_memory(&self, framework_memory: &str) -> AdapterResult<SemanticMemoryConfig> {
        if framework_memory.is_empty() {
            return Err(AdapterError::MemoryMappingError(
                "Empty memory definition".to_string(),
            ));
        }

        let memory_id = format!("autogen-mem-{}", ulid::Ulid::new());
        Ok(SemanticMemoryConfig {
            memory_id,
            memory_type: "conversation_history".to_string(),
            capacity_tokens: 200000,
            serialization_format: "json".to_string(),
        })
    }

    fn map_tool(&self, framework_tool: &str) -> AdapterResult<ToolBindingConfig> {
        if framework_tool.is_empty() {
            return Err(AdapterError::ToolBindingError(
                "Empty tool definition".to_string(),
            ));
        }
        self.map_function(framework_tool)
    }

    fn map_channel(&self, framework_comm: &str) -> AdapterResult<SemanticChannelConfig> {
        if framework_comm.is_empty() {
            return Err(AdapterError::ChannelMappingError(
                "Empty communication definition".to_string(),
            ));
        }
        self.map_conversation(framework_comm)
    }

    fn framework_type(&self) -> FrameworkType {
        FrameworkType::AutoGen
    }

    fn supported_versions(&self) -> &str {
        ">=0.2.0,<1.0.0"
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
    fn test_autogen_adapter_creation() {
        let adapter = AutoGenAdapter::new();
        assert_eq!(adapter.framework_type(), FrameworkType::AutoGen);
    }

    #[test]
    fn test_autogen_translate_to_ct() {
        let adapter = AutoGenAdapter::new();
        let result = adapter.translate_to_ct("agent definition");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.timeout_ms, 120000);
    }

    #[test]
    fn test_autogen_map_memory() {
        let adapter = AutoGenAdapter::new();
        let result = adapter.map_memory("conversation history");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.memory_type, "conversation_history");
    }

    #[test]
    fn test_autogen_map_tool() {
        let adapter = AutoGenAdapter::new();
        let result = adapter.map_tool("function definition");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.name, "AutoGenFunction");
    }

    #[test]
    fn test_autogen_map_channel() {
        let adapter = AutoGenAdapter::new();
        let result = adapter.map_channel("agent1, agent2");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.communication_pattern, "turn-taking");
    }

    #[test]
    fn test_autogen_supported_versions() {
        let adapter = AutoGenAdapter::new();
        let versions = adapter.supported_versions();
        assert!(versions.contains("0.2.0"));
    }

    #[test]
    fn test_autogen_default() {
        let adapter = AutoGenAdapter::default();
        assert_eq!(adapter.framework_type(), FrameworkType::AutoGen);
    }
}
