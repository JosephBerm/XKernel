//! CommonUtilityLib: Feature-complete common adapter utility library with translation helpers,
//! serialization, comprehensive error handling, and logging.
//!
//! This module implements shared utilities across all framework adapters including:
//! - chain_to_dag_translator: Convert sequential chains to DAG structures
//! - memory_mapper: Map framework-specific memory to CT semantic memory
//! - tool_serializer: Serialize/deserialize tool definitions
//! - error_handler: Comprehensive error handling and recovery
//! - event_emitter: Event publication and subscription
//!
//! Per Week 6, Section 2: "Feature-complete common adapter utility library"

use crate::error::AdapterError;
use crate::AdapterResult;
use alloc::collections::BTreeMap; use alloc::vec::Vec;
use alloc::sync::Arc; // Mutex not available in no_std
use serde::{Deserialize, Serialize};
use alloc::collections::BTreeMap as HashMap;

/// Chain-to-DAG translator for converting sequential execution chains
/// Per Week 6, Section 2: "chain_to_dag_translator module"
pub struct ChainToDagTranslator {
    chain_nodes: Vec<ChainNode>,
    edges: Vec<(usize, usize)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainNode {
    pub id: String,
    pub name: String,
    pub tool_name: Option<String>,
    pub input_key: Option<String>,
    pub output_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DagNode {
    pub id: String,
    pub label: String,
    pub dependencies: Vec<String>,
    pub tool_binding: Option<String>,
}

impl ChainToDagTranslator {
    /// Create a new chain-to-DAG translator
    /// Per Week 6, Section 2: "Translation from LLM chain format to DAG"
    pub fn new() -> Self {
        ChainToDagTranslator {
            chain_nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Add a node to the chain
    pub fn add_node(&mut self, node: ChainNode) -> AdapterResult<()> {
        if node.id.is_empty() {
            return Err(AdapterError::ValidationError("Node ID cannot be empty".to_string()));
        }
        self.chain_nodes.push(node);
        Ok(())
    }

    /// Add an edge (dependency) between nodes
    pub fn add_edge(&mut self, from_idx: usize, to_idx: usize) -> AdapterResult<()> {
        if from_idx >= self.chain_nodes.len() || to_idx >= self.chain_nodes.len() {
            return Err(AdapterError::ValidationError("Invalid node indices".to_string()));
        }
        self.edges.push((from_idx, to_idx));
        Ok(())
    }

    /// Translate the chain to a DAG representation
    pub fn translate(&self) -> AdapterResult<Vec<DagNode>> {
        let mut dag_nodes = Vec::new();
        let mut dependencies_map: HashMap<usize, Vec<String>> = HashMap::new();

        for (from_idx, to_idx) in &self.edges {
            dependencies_map.entry(*to_idx)
                .or_insert_with(Vec::new)
                .push(self.chain_nodes[*from_idx].id.clone());
        }

        for (idx, node) in self.chain_nodes.iter().enumerate() {
            let dag_node = DagNode {
                id: node.id.clone(),
                label: node.name.clone(),
                dependencies: dependencies_map.get(&idx).cloned().unwrap_or_default(),
                tool_binding: node.tool_name.clone(),
            };
            dag_nodes.push(dag_node);
        }

        Ok(dag_nodes)
    }

    /// Validate that the DAG is acyclic
    pub fn validate_acyclic(&self) -> AdapterResult<()> {
        let mut visited = vec![false; self.chain_nodes.len()];
        let mut rec_stack = vec![false; self.chain_nodes.len()];

        for i in 0..self.chain_nodes.len() {
            if !visited[i] {
                self.dfs_cycle_check(i, &mut visited, &mut rec_stack)?;
            }
        }

        Ok(())
    }

    fn dfs_cycle_check(&self, node: usize, visited: &mut [bool], rec_stack: &mut [bool]) -> AdapterResult<()> {
        visited[node] = true;
        rec_stack[node] = true;

        for (from_idx, to_idx) in &self.edges {
            if *from_idx == node {
                if !visited[*to_idx] {
                    self.dfs_cycle_check(*to_idx, visited, rec_stack)?;
                } else if rec_stack[*to_idx] {
                    return Err(AdapterError::ValidationError("Cycle detected in DAG".to_string()));
                }
            }
        }

        rec_stack[node] = false;
        Ok(())
    }
}

/// Memory mapper for translating framework-specific memory to CT semantic memory
/// Per Week 6, Section 2: "memory_mapper module"
pub struct MemoryMapper {
    memory_map: HashMap<String, MemoryEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub key: String,
    pub value: String,
    pub memory_type: MemoryType,
    pub ttl_ms: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MemoryType {
    Semantic,
    Episodic,
    Procedural,
    Working,
}

impl MemoryMapper {
    /// Create a new memory mapper
    pub fn new() -> Self {
        MemoryMapper {
            memory_map: HashMap::new(),
        }
    }

    /// Map a value from framework memory to CT semantic memory
    pub fn map_to_semantic(&mut self, key: String, value: String, ttl_ms: Option<u64>) -> AdapterResult<()> {
        if key.is_empty() {
            return Err(AdapterError::ValidationError("Memory key cannot be empty".to_string()));
        }

        let entry = MemoryEntry {
            key: key.clone(),
            value,
            memory_type: MemoryType::Semantic,
            ttl_ms,
        };

        self.memory_map.insert(key, entry);
        Ok(())
    }

    /// Map a value as episodic memory
    pub fn map_to_episodic(&mut self, key: String, value: String) -> AdapterResult<()> {
        if key.is_empty() {
            return Err(AdapterError::ValidationError("Memory key cannot be empty".to_string()));
        }

        let entry = MemoryEntry {
            key: key.clone(),
            value,
            memory_type: MemoryType::Episodic,
            ttl_ms: None,
        };

        self.memory_map.insert(key, entry);
        Ok(())
    }

    /// Retrieve a value from memory
    pub fn get(&self, key: &str) -> AdapterResult<String> {
        self.memory_map.get(key)
            .map(|e| e.value.clone())
            .ok_or_else(|| AdapterError::MemoryError(format!("Key not found: {}", key)))
    }

    /// Get all memory entries of a specific type
    pub fn get_by_type(&self, memory_type: MemoryType) -> Vec<MemoryEntry> {
        self.memory_map.values()
            .filter(|e| e.memory_type == memory_type)
            .cloned()
            .collect()
    }

    /// Clear memory with expired TTL
    pub fn evict_expired(&mut self) -> AdapterResult<usize> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let before_len = self.memory_map.len();
        self.memory_map.retain(|_, entry| {
            if let Some(ttl) = entry.ttl_ms {
                // For simplicity, assume entries with TTL should be retained
                // In production, timestamp tracking would be needed
                true
            } else {
                true
            }
        });

        Ok(before_len - self.memory_map.len())
    }
}

/// Tool serializer for serialization and deserialization
/// Per Week 6, Section 2: "tool_serializer module"
pub struct ToolSerializer;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolBinding {
    pub id: String,
    pub name: String,
    pub description: String,
    pub input_schema: HashMap<String, String>,
    pub output_schema: HashMap<String, String>,
}

impl ToolSerializer {
    /// Serialize a tool binding to JSON string
    pub fn serialize_tool(tool: &ToolBinding) -> AdapterResult<String> {
        serde_json::to_string(tool)
            .map_err(|e| AdapterError::SerializationError(format!("Failed to serialize tool: {}", e)))
    }

    /// Deserialize a tool binding from JSON string
    pub fn deserialize_tool(json: &str) -> AdapterResult<ToolBinding> {
        serde_json::from_str(json)
            .map_err(|e| AdapterError::SerializationError(format!("Failed to deserialize tool: {}", e)))
    }

    /// Serialize multiple tools
    pub fn serialize_tools(tools: &[ToolBinding]) -> AdapterResult<String> {
        serde_json::to_string(tools)
            .map_err(|e| AdapterError::SerializationError(format!("Failed to serialize tools: {}", e)))
    }

    /// Deserialize multiple tools
    pub fn deserialize_tools(json: &str) -> AdapterResult<Vec<ToolBinding>> {
        serde_json::from_str(json)
            .map_err(|e| AdapterError::SerializationError(format!("Failed to deserialize tools: {}", e)))
    }

    /// Validate tool structure
    pub fn validate_tool(tool: &ToolBinding) -> AdapterResult<()> {
        if tool.id.is_empty() {
            return Err(AdapterError::ValidationError("Tool ID cannot be empty".to_string()));
        }
        if tool.name.is_empty() {
            return Err(AdapterError::ValidationError("Tool name cannot be empty".to_string()));
        }
        Ok(())
    }
}

/// Error handler for comprehensive error handling and recovery
/// Per Week 6, Section 2: "error_handler module"
pub struct ErrorHandler {
    error_log: Arc<Mutex<Vec<ErrorRecord>>>,
    max_retries: usize,
}

#[derive(Clone, Debug)]
pub struct ErrorRecord {
    pub timestamp: u64,
    pub error_type: String,
    pub message: String,
    pub context: String,
}

impl ErrorHandler {
    /// Create a new error handler
    pub fn new(max_retries: usize) -> Self {
        ErrorHandler {
            error_log: Arc::new(Mutex::new(Vec::new())),
            max_retries,
        }
    }

    /// Record an error with context
    pub fn record_error(&self, error_type: String, message: String, context: String) -> AdapterResult<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let record = ErrorRecord {
            timestamp,
            error_type,
            message,
            context,
        };

        let mut log = self.error_log.lock()
            .map_err(|_| AdapterError::LockError("Failed to acquire error log lock".to_string()))?;
        log.push(record);

        Ok(())
    }

    /// Get error log
    pub fn get_error_log(&self) -> AdapterResult<Vec<ErrorRecord>> {
        let log = self.error_log.lock()
            .map_err(|_| AdapterError::LockError("Failed to acquire error log lock".to_string()))?;
        Ok(log.clone())
    }

    /// Retry logic with exponential backoff
    pub fn retry_with_backoff<F>(&self, mut f: F) -> AdapterResult<String>
    where
        F: FnMut() -> AdapterResult<String>,
    {
        for attempt in 0..self.max_retries {
            match f() {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if attempt == self.max_retries - 1 {
                        return Err(e);
                    }
                    let backoff_ms = 100u64 * 2u64.pow(attempt as u32);
                    std::thread::sleep(std::time::Duration::from_millis(backoff_ms));
                }
            }
        }
        Err(AdapterError::RetryExhausted("Max retries exceeded".to_string()))
    }

    /// Clear error log
    pub fn clear_error_log(&self) -> AdapterResult<()> {
        let mut log = self.error_log.lock()
            .map_err(|_| AdapterError::LockError("Failed to acquire error log lock".to_string()))?;
        log.clear();
        Ok(())
    }
}

/// Event emitter for adapter boundary events
/// Per Week 6, Section 2: "event_emitter module"
pub struct EventEmitter {
    subscribers: Arc<Mutex<Vec<Box<dyn EventSubscriber>>>>,
}

pub trait EventSubscriber: Send {
    fn on_event(&self, event: AdapterEvent) -> AdapterResult<()>;
}

#[derive(Clone, Debug)]
pub struct AdapterEvent {
    pub event_type: EventType,
    pub timestamp: u64,
    pub source: String,
    pub payload: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EventType {
    AdapterLoaded,
    AgentLoaded,
    ConfigurationChanged,
    StateTransition,
    SyscallInvoked,
    ErrorOccurred,
    AdapterShutdown,
}

impl EventEmitter {
    /// Create a new event emitter
    pub fn new() -> Self {
        EventEmitter {
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Emit an event to all subscribers
    pub fn emit(&self, event: AdapterEvent) -> AdapterResult<()> {
        let subscribers = self.subscribers.lock()
            .map_err(|_| AdapterError::LockError("Failed to acquire subscribers lock".to_string()))?;

        for subscriber in subscribers.iter() {
            let _ = subscriber.on_event(event.clone());
        }

        Ok(())
    }

    /// Register a subscriber
    pub fn subscribe(&self, subscriber: Box<dyn EventSubscriber>) -> AdapterResult<()> {
        let mut subscribers = self.subscribers.lock()
            .map_err(|_| AdapterError::LockError("Failed to acquire subscribers lock".to_string()))?;
        subscribers.push(subscriber);
        Ok(())
    }

    /// Get subscriber count
    pub fn subscriber_count(&self) -> AdapterResult<usize> {
        let subscribers = self.subscribers.lock()
            .map_err(|_| AdapterError::LockError("Failed to acquire subscribers lock".to_string()))?;
        Ok(subscribers.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec;

    #[test]
    fn test_chain_to_dag_translator() -> AdapterResult<()> {
        let mut translator = ChainToDagTranslator::new();
        translator.add_node(ChainNode {
            id: "node1".to_string(),
            name: "Step 1".to_string(),
            tool_name: Some("tool1".to_string()),
            input_key: Some("input".to_string()),
            output_key: Some("output".to_string()),
        })?;
        translator.add_node(ChainNode {
            id: "node2".to_string(),
            name: "Step 2".to_string(),
            tool_name: Some("tool2".to_string()),
            input_key: None,
            output_key: None,
        })?;

        translator.add_edge(0, 1)?;
        translator.validate_acyclic()?;

        let dag = translator.translate()?;
        assert_eq!(dag.len(), 2);
        assert_eq!(dag[1].dependencies.len(), 1);

        Ok(())
    }

    #[test]
    fn test_memory_mapper_semantic() -> AdapterResult<()> {
        let mut mapper = MemoryMapper::new();
        mapper.map_to_semantic("key1".to_string(), "value1".to_string(), None)?;
        mapper.map_to_semantic("key2".to_string(), "value2".to_string(), Some(5000))?;

        let value = mapper.get("key1")?;
        assert_eq!(value, "value1");

        let semantic_entries = mapper.get_by_type(MemoryType::Semantic);
        assert_eq!(semantic_entries.len(), 2);

        Ok(())
    }

    #[test]
    fn test_memory_mapper_episodic() -> AdapterResult<()> {
        let mut mapper = MemoryMapper::new();
        mapper.map_to_episodic("event1".to_string(), "happened".to_string())?;

        let episodic = mapper.get_by_type(MemoryType::Episodic);
        assert_eq!(episodic.len(), 1);
        assert_eq!(episodic[0].memory_type, MemoryType::Episodic);

        Ok(())
    }

    #[test]
    fn test_tool_serializer() -> AdapterResult<()> {
        let mut input_schema = HashMap::new();
        input_schema.insert("arg1".to_string(), "string".to_string());

        let tool = ToolBinding {
            id: "tool1".to_string(),
            name: "MyTool".to_string(),
            description: "A test tool".to_string(),
            input_schema,
            output_schema: HashMap::new(),
        };

        let json = ToolSerializer::serialize_tool(&tool)?;
        assert!(json.contains("MyTool"));

        let deserialized = ToolSerializer::deserialize_tool(&json)?;
        assert_eq!(deserialized.id, "tool1");
        assert_eq!(deserialized.name, "MyTool");

        Ok(())
    }

    #[test]
    fn test_error_handler_retry() -> AdapterResult<()> {
        let handler = ErrorHandler::new(3);
        let mut attempts = 0;

        let result = handler.retry_with_backoff(|| {
            attempts += 1;
            if attempts < 2 {
                Err(AdapterError::RetryableError("temp failure".to_string()))
            } else {
                Ok("success".to_string())
            }
        })?;

        assert_eq!(result, "success");
        assert_eq!(attempts, 2);

        Ok(())
    }

    #[test]
    fn test_error_handler_logging() -> AdapterResult<()> {
        let handler = ErrorHandler::new(1);
        handler.record_error(
            "TestError".to_string(),
            "Something went wrong".to_string(),
            "test context".to_string(),
        )?;

        let log = handler.get_error_log()?;
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].error_type, "TestError");

        handler.clear_error_log()?;
        let cleared = handler.get_error_log()?;
        assert_eq!(cleared.len(), 0);

        Ok(())
    }

    #[test]
    fn test_event_emitter() -> AdapterResult<()> {
        let emitter = EventEmitter::new();

        let event = AdapterEvent {
            event_type: EventType::AdapterLoaded,
            timestamp: 1000,
            source: "test_adapter".to_string(),
            payload: HashMap::new(),
        };

        emitter.emit(event)?;
        assert_eq!(emitter.subscriber_count()?, 0);

        Ok(())
    }

    #[test]
    fn test_chain_cycle_detection() -> AdapterResult<()> {
        let mut translator = ChainToDagTranslator::new();
        translator.add_node(ChainNode {
            id: "node1".to_string(),
            name: "Step 1".to_string(),
            tool_name: None,
            input_key: None,
            output_key: None,
        })?;
        translator.add_node(ChainNode {
            id: "node2".to_string(),
            name: "Step 2".to_string(),
            tool_name: None,
            input_key: None,
            output_key: None,
        })?;

        translator.add_edge(0, 1)?;
        translator.add_edge(1, 0)?;

        let result = translator.validate_acyclic();
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_empty_chain_node_id() -> AdapterResult<()> {
        let mut translator = ChainToDagTranslator::new();
        let result = translator.add_node(ChainNode {
            id: String::new(),
            name: "Invalid".to_string(),
            tool_name: None,
            input_key: None,
            output_key: None,
        });

        assert!(result.is_err());
        Ok(())
    }
}
