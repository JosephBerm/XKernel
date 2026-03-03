// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! LangChain adapter implementation.
//!
//! Provides translation between LangChain framework concepts and CSCI primitives,
//! including agent spawning, event translation, and lifecycle management.
//! - Tool → ToolBinding (Full fidelity)
//! - Memory → SemanticMemory (Partial fidelity)
//! - Agent → Agent (Full fidelity)
//! - AgentExecutor → AgentCrew (Partial fidelity, coordinator role)

use crate::adapter::{
    CognitiveTaskConfig, IFrameworkAdapter, SemanticChannelConfig, SemanticMemoryConfig,
    ToolBindingConfig, TranslationResult,
};
use crate::framework_type::FrameworkType;
use crate::error::AdapterError;
use crate::AdapterResult;

/// LangChain framework adapter.
/// Sec 4.2: Framework Adapter Implementation
#[derive(Debug, Clone)]
pub struct LangChainAdapter {
    /// Minimum supported version
    min_version: String,
    /// Maximum supported version
    max_version: String,
}

impl LangChainAdapter {
    /// Creates a new LangChain adapter instance.
    /// Sec 4.2: Adapter Instantiation
    pub fn new() -> Self {
        LangChainAdapter {
            min_version: "0.0.1".to_string(),
            max_version: "0.2.x".to_string(),
        }
    }

    /// Maps a LangChain chain to CognitiveTask.
    /// Sec 4.3: Chain Mapping
    fn map_chain(&self, chain_def: &str) -> AdapterResult<CognitiveTaskConfig> {
        // Parse chain definition (simplified for demonstration)
        // In production, this would deserialize from JSON/YAML and validate schema
        let task_id = format!("lc-chain-{}", ulid::Ulid::new());

        Ok(CognitiveTaskConfig {
            task_id,
            name: "LangChainChain".to_string(),
            objective: chain_def.to_string(),
            timeout_ms: 30000,
            is_mandatory: false,
        })
    }

    /// Maps a LangChain tool to ToolBinding.
    /// Sec 4.3: Tool Mapping
    fn map_tool_binding(&self, tool_def: &str) -> AdapterResult<ToolBindingConfig> {
        let tool_id = format!("lc-tool-{}", ulid::Ulid::new());

        Ok(ToolBindingConfig {
            tool_id,
            name: "LangChainTool".to_string(),
            description: tool_def.to_string(),
            input_schema: "{}".to_string(),
            output_schema: "{}".to_string(),
            requires_authorization: false,
        })
    }

    /// Maps LangChain memory to SemanticMemory.
    /// Sec 4.3: Memory Mapping (Partial Fidelity)
    fn map_memory_impl(&self, memory_def: &str) -> AdapterResult<SemanticMemoryConfig> {
        let memory_id = format!("lc-mem-{}", ulid::Ulid::new());

        Ok(SemanticMemoryConfig {
            memory_id,
            memory_type: "conversation_buffer".to_string(),
            capacity_tokens: 10000,
            serialization_format: "json".to_string(),
        })
    }
}

impl Default for LangChainAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl IFrameworkAdapter for LangChainAdapter {
    fn translate_to_ct(&self, framework_task: &str) -> AdapterResult<CognitiveTaskConfig> {
        if framework_task.is_empty() {
            return Err(AdapterError::TranslationError(
                "Empty framework task definition".to_string(),
            ));
        }
        self.map_chain(framework_task)
    }

    fn translate_from_ct(&self, task_id: &str, result: &str) -> AdapterResult<TranslationResult> {
        Ok(TranslationResult {
            artifact_id: task_id.to_string(),
            artifact_type: "langchain_result".to_string(),
            success: true,
            fidelity: "full".to_string(),
            translation_notes: "LangChain result translated from CSCI CognitiveTask".to_string(),
        })
    }

    fn map_memory(&self, framework_memory: &str) -> AdapterResult<SemanticMemoryConfig> {
        if framework_memory.is_empty() {
            return Err(AdapterError::MemoryMappingError(
                "Empty memory definition".to_string(),
            ));
        }
        self.map_memory_impl(framework_memory)
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

        let channel_id = format!("lc-ch-{}", ulid::Ulid::new());
        Ok(SemanticChannelConfig {
            channel_id,
            name: "LangChainChannel".to_string(),
            participants: "agent".to_string(),
            communication_pattern: "request-reply".to_string(),
            is_persistent: false,
        })
    }

    fn framework_type(&self) -> FrameworkType {
        FrameworkType::LangChain
    }

    fn supported_versions(&self) -> &str {
        ">=0.0.1,<0.3.0"
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
    fn test_langchain_adapter_creation() {
        let adapter = LangChainAdapter::new();
        assert_eq!(adapter.framework_type(), FrameworkType::LangChain);
    }

    #[test]
    fn test_langchain_translate_to_ct() {
        let adapter = LangChainAdapter::new();
        let result = adapter.translate_to_ct("chain definition");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.timeout_ms, 30000);
    }

    #[test]
    fn test_langchain_translate_to_ct_empty() {
        let adapter = LangChainAdapter::new();
        let result = adapter.translate_to_ct("");
        assert!(result.is_err());
    }

    #[test]
    fn test_langchain_map_memory() {
        let adapter = LangChainAdapter::new();
        let result = adapter.map_memory("memory config");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.memory_type, "conversation_buffer");
    }

    #[test]
    fn test_langchain_map_tool() {
        let adapter = LangChainAdapter::new();
        let result = adapter.map_tool("tool definition");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.name, "LangChainTool");
    }

    #[test]
    fn test_langchain_map_channel() {
        let adapter = LangChainAdapter::new();
        let result = adapter.map_channel("channel config");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.communication_pattern, "request-reply");
    }

    #[test]
    fn test_langchain_supported_versions() {
        let adapter = LangChainAdapter::new();
        let versions = adapter.supported_versions();
        assert!(versions.contains("0.0.1"));
    }

    #[test]
    fn test_langchain_is_compatible() {
        let adapter = LangChainAdapter::new();
        assert!(adapter.is_compatible("any artifact"));
    }

    #[test]
    fn test_langchain_default() {
        let adapter = LangChainAdapter::default();
        assert_eq!(adapter.framework_type(), FrameworkType::LangChain);
    }
}
