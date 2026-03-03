//! LangChainAdapterV2: 30% implementation of LangChain adapter with core translation logic.
//!
//! This module provides adapters to translate LangChain framework constructs into Cognitive
//! Substrate runtime primitives:
//! - BasicChainTranslator: Convert sequential chains to CT DAG format
//! - SimpleMemoryMapper: Map LangChain memory to CT semantic memory
//! - LangChainToolAdapter: Wrap LangChain tools as CT ToolBindings
//!
//! Per Week 6, Section 4: "30% implementation of LangChain adapter"

use crate::error::AdapterError;
use crate::AdapterResult;
use crate::common_utility_lib::{ChainToDagTranslator, ChainNode, DagNode, MemoryMapper, MemoryType, ToolBinding};
use alloc::collections::BTreeMap; use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use alloc::collections::BTreeMap as HashMap;

/// LangChain chain representation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LangChainChain {
    pub id: String,
    pub name: String,
    pub steps: Vec<ChainStep>,
    pub metadata: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainStep {
    pub index: usize,
    pub name: String,
    pub step_type: String,
    pub config: HashMap<String, String>,
}

/// LangChain memory representation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LangChainMemory {
    pub memory_type: String,
    pub buffer: HashMap<String, String>,
    pub input_key: String,
    pub output_key: String,
}

/// LangChain tool representation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LangChainTool {
    pub name: String,
    pub description: String,
    pub func: String,
    pub input_schema: HashMap<String, String>,
}

/// BasicChainTranslator: Convert sequential LangChain chains to CT DAG format
/// Per Week 6, Section 4: "BasicChainTranslator for sequential chains"
pub struct BasicChainTranslator {
    chain: LangChainChain,
}

impl BasicChainTranslator {
    /// Create a new BasicChainTranslator from a LangChain chain
    pub fn new(chain: LangChainChain) -> Self {
        BasicChainTranslator { chain }
    }

    /// Translate LangChain chain to DAG representation
    pub fn translate(&self) -> AdapterResult<Vec<DagNode>> {
        let mut translator = ChainToDagTranslator::new();

        // Convert LangChain steps to chain nodes
        for step in &self.chain.steps {
            let node = ChainNode {
                id: format!("step_{}", step.index),
                name: step.name.clone(),
                tool_name: Some(step.step_type.clone()),
                input_key: Some("input".to_string()),
                output_key: Some("output".to_string()),
            };
            translator.add_node(node)?;
        }

        // Add sequential edges: step_i -> step_i+1
        for i in 0..self.chain.steps.len().saturating_sub(1) {
            translator.add_edge(i, i + 1)?;
        }

        // Validate and translate
        translator.validate_acyclic()?;
        translator.translate()
    }

    /// Get chain metadata
    pub fn get_metadata(&self) -> HashMap<String, String> {
        self.chain.metadata.clone()
    }

    /// Validate chain structure
    pub fn validate(&self) -> AdapterResult<()> {
        if self.chain.id.is_empty() {
            return Err(AdapterError::ValidationError("Chain ID cannot be empty".to_string()));
        }

        if self.chain.steps.is_empty() {
            return Err(AdapterError::ValidationError("Chain must have at least one step".to_string()));
        }

        for (idx, step) in self.chain.steps.iter().enumerate() {
            if step.name.is_empty() {
                return Err(AdapterError::ValidationError(
                    format!("Step {} has empty name", idx),
                ));
            }
        }

        Ok(())
    }

    /// Get number of steps
    pub fn step_count(&self) -> usize {
        self.chain.steps.len()
    }
}

/// SimpleMemoryMapper: Map LangChain memory to CT semantic memory
/// Per Week 6, Section 4: "SimpleMemoryMapper for LangChain memory"
pub struct SimpleMemoryMapper {
    memory_mapper: MemoryMapper,
    lc_memory: LangChainMemory,
}

impl SimpleMemoryMapper {
    /// Create a new SimpleMemoryMapper from LangChain memory
    pub fn new(lc_memory: LangChainMemory) -> Self {
        SimpleMemoryMapper {
            memory_mapper: MemoryMapper::new(),
            lc_memory,
        }
    }

    /// Map all LangChain memory to CT semantic memory
    pub fn map_all(&mut self) -> AdapterResult<()> {
        for (key, value) in &self.lc_memory.buffer {
            self.memory_mapper.map_to_semantic(
                format!("lc_{}", key),
                value.clone(),
                None,
            )?;
        }
        Ok(())
    }

    /// Map a specific LangChain memory entry
    pub fn map_entry(&mut self, key: String, value: String) -> AdapterResult<()> {
        let ct_key = format!("lc_mapped_{}", key);
        self.memory_mapper.map_to_semantic(ct_key, value, None)?;
        Ok(())
    }

    /// Get mapped value
    pub fn get(&self, key: &str) -> AdapterResult<String> {
        let ct_key = format!("lc_{}", key);
        self.memory_mapper.get(&ct_key)
    }

    /// Map conversation history to episodic memory
    pub fn map_conversation_history(&mut self, conversation: Vec<(String, String)>) -> AdapterResult<()> {
        for (idx, (speaker, message)) in conversation.iter().enumerate() {
            let key = format!("conversation_{}_{}", idx, speaker);
            self.memory_mapper.map_to_episodic(key, message.clone())?;
        }
        Ok(())
    }

    /// Get semantic memory entries
    pub fn get_semantic_entries(&self) -> Vec<String> {
        let entries = self.memory_mapper.get_by_type(MemoryType::Semantic);
        entries.iter().map(|e| e.value.clone()).collect()
    }

    /// Get episodic memory count
    pub fn get_episodic_count(&self) -> usize {
        let entries = self.memory_mapper.get_by_type(MemoryType::Episodic);
        entries.len()
    }
}

/// LangChainToolAdapter: Wrap LangChain tools as CT ToolBindings
/// Per Week 6, Section 4: "LangChainToolAdapter for tool integration"
pub struct LangChainToolAdapter {
    tool: LangChainTool,
}

impl LangChainToolAdapter {
    /// Create a new LangChainToolAdapter from a LangChain tool
    pub fn new(tool: LangChainTool) -> Self {
        LangChainToolAdapter { tool }
    }

    /// Convert LangChain tool to CT ToolBinding
    pub fn to_tool_binding(&self) -> AdapterResult<ToolBinding> {
        self.validate()?;

        Ok(ToolBinding {
            id: format!("lc_tool_{}", self.tool.name),
            name: self.tool.name.clone(),
            description: self.tool.description.clone(),
            input_schema: self.tool.input_schema.clone(),
            output_schema: {
                let mut schema = HashMap::new();
                schema.insert("result".to_string(), "string".to_string());
                schema
            },
        })
    }

    /// Validate tool structure
    pub fn validate(&self) -> AdapterResult<()> {
        if self.tool.name.is_empty() {
            return Err(AdapterError::ValidationError("Tool name cannot be empty".to_string()));
        }

        if self.tool.func.is_empty() {
            return Err(AdapterError::ValidationError("Tool function cannot be empty".to_string()));
        }

        Ok(())
    }

    /// Get tool name
    pub fn get_name(&self) -> String {
        self.tool.name.clone()
    }

    /// Get tool description
    pub fn get_description(&self) -> String {
        self.tool.description.clone()
    }

    /// Simulate tool invocation
    pub fn invoke(&self, args: HashMap<String, String>) -> AdapterResult<String> {
        self.validate()?;

        let arg_str = args.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(", ");

        Ok(format!(
            "LangChain tool '{}' invoked with args: [{}]",
            self.tool.name, arg_str
        ))
    }

    /// Get input schema
    pub fn get_input_schema(&self) -> HashMap<String, String> {
        self.tool.input_schema.clone()
    }
}

/// LangChainAdapterContext: Manage complete chain execution context
pub struct LangChainAdapterContext {
    chain_translator: Option<BasicChainTranslator>,
    memory_mapper: Option<SimpleMemoryMapper>,
    tools: HashMap<String, LangChainToolAdapter>,
    execution_state: ExecutionState,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExecutionState {
    Uninitialized,
    ChainLoaded,
    MemoryLoaded,
    Ready,
    Executing,
    Complete,
}

impl LangChainAdapterContext {
    /// Create a new adapter context
    pub fn new() -> Self {
        LangChainAdapterContext {
            chain_translator: None,
            memory_mapper: None,
            tools: HashMap::new(),
            execution_state: ExecutionState::Uninitialized,
        }
    }

    /// Load a LangChain chain
    pub fn load_chain(&mut self, chain: LangChainChain) -> AdapterResult<()> {
        let translator = BasicChainTranslator::new(chain);
        translator.validate()?;

        self.chain_translator = Some(translator);
        self.execution_state = ExecutionState::ChainLoaded;

        Ok(())
    }

    /// Load LangChain memory
    pub fn load_memory(&mut self, memory: LangChainMemory) -> AdapterResult<()> {
        let mut mapper = SimpleMemoryMapper::new(memory);
        mapper.map_all()?;

        self.memory_mapper = Some(mapper);
        self.execution_state = ExecutionState::MemoryLoaded;

        Ok(())
    }

    /// Register a LangChain tool
    pub fn register_tool(&mut self, tool: LangChainTool) -> AdapterResult<()> {
        let adapter = LangChainToolAdapter::new(tool.clone());
        adapter.validate()?;

        self.tools.insert(tool.name.clone(), adapter);

        Ok(())
    }

    /// Prepare for execution
    pub fn prepare_execution(&mut self) -> AdapterResult<()> {
        if self.chain_translator.is_none() {
            return Err(AdapterError::StateError("Chain not loaded".to_string()));
        }

        self.execution_state = ExecutionState::Ready;
        Ok(())
    }

    /// Simulate execution
    pub fn execute(&mut self) -> AdapterResult<String> {
        if self.execution_state != ExecutionState::Ready {
            return Err(AdapterError::StateError(
                format!("Cannot execute in state: {:?}", self.execution_state),
            ));
        }

        self.execution_state = ExecutionState::Executing;

        let result = if let Some(translator) = &self.chain_translator {
            format!("Executed chain with {} steps", translator.step_count())
        } else {
            "No chain to execute".to_string()
        };

        self.execution_state = ExecutionState::Complete;
        Ok(result)
    }

    /// Get execution state
    pub fn get_state(&self) -> ExecutionState {
        self.execution_state.clone()
    }

    /// Get tool count
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::vec;

    #[test]
    fn test_basic_chain_translator() -> AdapterResult<()> {
        let chain = LangChainChain {
            id: "chain1".to_string(),
            name: "Test Chain".to_string(),
            steps: vec![
                ChainStep {
                    index: 0,
                    name: "Step 1".to_string(),
                    step_type: "tool".to_string(),
                    config: HashMap::new(),
                },
                ChainStep {
                    index: 1,
                    name: "Step 2".to_string(),
                    step_type: "tool".to_string(),
                    config: HashMap::new(),
                },
            ],
            metadata: HashMap::new(),
        };

        let translator = BasicChainTranslator::new(chain);
        translator.validate()?;

        let dag = translator.translate()?;
        assert_eq!(dag.len(), 2);

        Ok(())
    }

    #[test]
    fn test_simple_memory_mapper() -> AdapterResult<()> {
        let mut buffer = HashMap::new();
        buffer.insert("key1".to_string(), "value1".to_string());
        buffer.insert("key2".to_string(), "value2".to_string());

        let memory = LangChainMemory {
            memory_type: "buffer".to_string(),
            buffer,
            input_key: "input".to_string(),
            output_key: "output".to_string(),
        };

        let mut mapper = SimpleMemoryMapper::new(memory);
        mapper.map_all()?;

        let value = mapper.get("key1")?;
        assert_eq!(value, "value1");

        Ok(())
    }

    #[test]
    fn test_langchain_tool_adapter() -> AdapterResult<()> {
        let tool = LangChainTool {
            name: "calculator".to_string(),
            description: "A simple calculator".to_string(),
            func: "calculate".to_string(),
            input_schema: {
                let mut schema = HashMap::new();
                schema.insert("expr".to_string(), "string".to_string());
                schema
            },
        };

        let adapter = LangChainToolAdapter::new(tool);
        adapter.validate()?;

        let binding = adapter.to_tool_binding()?;
        assert_eq!(binding.name, "calculator");

        Ok(())
    }

    #[test]
    fn test_adapter_context_workflow() -> AdapterResult<()> {
        let mut context = LangChainAdapterContext::new();

        let chain = LangChainChain {
            id: "chain1".to_string(),
            name: "Test Chain".to_string(),
            steps: vec![ChainStep {
                index: 0,
                name: "Step 1".to_string(),
                step_type: "tool".to_string(),
                config: HashMap::new(),
            }],
            metadata: HashMap::new(),
        };

        context.load_chain(chain)?;
        assert_eq!(context.get_state(), ExecutionState::ChainLoaded);

        let memory = LangChainMemory {
            memory_type: "buffer".to_string(),
            buffer: HashMap::new(),
            input_key: "input".to_string(),
            output_key: "output".to_string(),
        };

        context.load_memory(memory)?;
        assert_eq!(context.get_state(), ExecutionState::MemoryLoaded);

        context.prepare_execution()?;
        assert_eq!(context.get_state(), ExecutionState::Ready);

        let result = context.execute()?;
        assert!(result.contains("Executed chain"));

        Ok(())
    }

    #[test]
    fn test_chain_validation_empty_id() {
        let chain = LangChainChain {
            id: String::new(),
            name: "Invalid".to_string(),
            steps: vec![],
            metadata: HashMap::new(),
        };

        let translator = BasicChainTranslator::new(chain);
        let result = translator.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_conversation_mapping() -> AdapterResult<()> {
        let memory = LangChainMemory {
            memory_type: "buffer".to_string(),
            buffer: HashMap::new(),
            input_key: "input".to_string(),
            output_key: "output".to_string(),
        };

        let mut mapper = SimpleMemoryMapper::new(memory);
        let conversation = vec![
            ("user".to_string(), "Hello".to_string()),
            ("assistant".to_string(), "Hi there!".to_string()),
        ];

        mapper.map_conversation_history(conversation)?;
        assert_eq!(mapper.get_episodic_count(), 2);

        Ok(())
    }

    #[test]
    fn test_tool_invocation() -> AdapterResult<()> {
        let tool = LangChainTool {
            name: "search".to_string(),
            description: "Search tool".to_string(),
            func: "search_func".to_string(),
            input_schema: {
                let mut schema = HashMap::new();
                schema.insert("query".to_string(), "string".to_string());
                schema
            },
        };

        let adapter = LangChainToolAdapter::new(tool);
        let mut args = HashMap::new();
        args.insert("query".to_string(), "python".to_string());

        let result = adapter.invoke(args)?;
        assert!(result.contains("search"));

        Ok(())
    }
}
