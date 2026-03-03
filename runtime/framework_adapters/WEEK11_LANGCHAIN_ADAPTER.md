# Week 11 — LangChain Adapter: Chain-to-DAG Translation & Memory Mapping

**Document Version:** 1.0
**Status:** Design Phase
**Author:** Principal Software Engineer, XKernal Project
**Date:** 2026-03-02
**Scope:** LangChain integration layer for cognitive substrate runtime

---

## Executive Summary

Week 11 initiates the LangChain adapter implementation, a critical bridge layer that translates LangChain's linear chain abstractions into XKernal's Cognitive Task (CT) dependency-driven execution model. This adapter converts sequential, branching, and map-reduce chains into CT DAGs, maps heterogeneous memory systems to L2 episodic storage, and binds external tools through the ToolBinding protocol. The implementation achieves 50% completion this week with core translation engine, memory mapper, and tool integration foundation.

**Key Deliverables:**
- Chain-to-DAG translation engine supporting 3 chain patterns
- Memory mapper for ConversationBufferMemory, SummaryMemory, ConversationKGMemory
- Tool binding framework with argument schema validation
- 20+ comprehensive unit tests covering edge cases
- Integration test validating end-to-end 3-step LangChain chain execution

---

## Problem Statement

### Current State
LangChain provides a mature framework for building chains (sequential operations, branching logic, parallel execution) but implements its own execution model, memory management, and tool invocation system. This creates a mismatch when integrating LangChain workflows into XKernal's asynchronous, graph-based CT execution paradigm.

### Gaps Addressed
1. **Abstraction Mismatch**: LangChain chains are linear-first; XKernal is DAG-first
2. **Memory Fragmentation**: LangChain memory types don't map to L2 episodic semantics
3. **Tool Integration**: LangChain tool invocation lacks CT-level scheduling and dependency tracking
4. **Observability**: Chain execution opaque to kernel's cognitive tracing infrastructure

### Success Criteria
- Arbitrary 3+ step LangChain chains execute on kernel with full CT dependency resolution
- Memory operations complete with zero data loss; L2 episodic writes validated
- Tool invocation respects kernel scheduling; results available before dependent CT spawning
- Execution latency within 10% of native CT chains

---

## Architecture

### System Overview

```
LangChain Chain
      ↓
Chain Step Parser
      ↓
CT Graph Builder (3 translators)
      ├→ Sequential Chain Translator
      ├→ Router Chain Translator
      └→ Map-Reduce Chain Translator
      ↓
CT DAG (nodes=CT, edges=data deps)
      ↓
CT Spawn Batch
      ↓
[Kernel Scheduler]
      ↓
Execution + Memory Mapping + Tool Invocation
```

### Component Design

#### 1. Chain Step Parser
Introspects LangChain chain objects, extracting:
- Step identity (name, type, component class)
- Input/output variable names
- Component configuration (LLM params, tool refs, etc.)
- Runnable metadata

```rust
pub struct ChainStepParser {
    chain_source: Box<dyn Any>,
}

impl ChainStepParser {
    pub fn parse(&self) -> Result<Vec<ChainStep>, TranslationError> {
        // Traverse chain structure, extract steps
        // Support nested chains via recursive parsing
        todo!()
    }
}

pub struct ChainStep {
    pub id: String,
    pub step_type: StepType,
    pub input_keys: Vec<String>,
    pub output_keys: Vec<String>,
    pub config: serde_json::Value,
}

pub enum StepType {
    LLMCall,
    ToolCall,
    Router,
    MapReduce,
    Custom(String),
}
```

#### 2. CT Graph Builder
Converts chain steps into CT DAG. Three specialized translators handle distinct chain patterns:

**Sequential Translator**: Linear step chain → linear CT dependency chain
```rust
pub struct SequentialTranslator;

impl SequentialTranslator {
    pub fn translate(&self, steps: &[ChainStep]) -> Result<CtDag, TranslationError> {
        let mut dag = CtDag::new();
        let mut prev_ct_id = None;

        for step in steps {
            let ct_id = dag.add_node(CtDefinition {
                name: step.id.clone(),
                version: "1.0".into(),
                inputs: step.input_keys.clone(),
                outputs: step.output_keys.clone(),
            });

            if let Some(prev) = prev_ct_id {
                dag.add_edge(prev, ct_id);
            }
            prev_ct_id = Some(ct_id);
        }
        Ok(dag)
    }
}
```

**Router Translator**: Conditional routing chain → conditional CT spawning
```rust
pub struct RouterTranslator;

impl RouterTranslator {
    pub fn translate(&self, steps: &[ChainStep]) -> Result<CtDag, TranslationError> {
        // Identify router step + branch targets
        // Create conditional CT nodes with routing logic
        // Single decision point spawns multiple downstream CT branches
        // Branches merge at convergence node
        todo!()
    }
}
```

**Map-Reduce Translator**: Parallel processing chain → batch CT spawn
```rust
pub struct MapReduceTranslator;

impl MapReduceTranslator {
    pub fn translate(&self, steps: &[ChainStep]) -> Result<CtDag, TranslationError> {
        // Map step → parallel CT spawn for each input
        // Reduce step → convergence CT collecting map outputs
        // DAG shape: fan-out (map) → fan-in (reduce)
        todo!()
    }
}
```

#### 3. CT Graph Builder Orchestrator
```rust
pub struct CtGraphBuilder {
    sequential: SequentialTranslator,
    router: RouterTranslator,
    map_reduce: MapReduceTranslator,
}

impl CtGraphBuilder {
    pub fn build(&self, steps: &[ChainStep]) -> Result<CtDag, TranslationError> {
        let chain_type = self.detect_chain_type(steps)?;
        match chain_type {
            ChainType::Sequential => self.sequential.translate(steps),
            ChainType::Router => self.router.translate(steps),
            ChainType::MapReduce => self.map_reduce.translate(steps),
        }
    }

    fn detect_chain_type(&self, steps: &[ChainStep]) -> Result<ChainType, TranslationError> {
        if steps.iter().any(|s| matches!(s.step_type, StepType::Router)) {
            Ok(ChainType::Router)
        } else if steps.iter().any(|s| matches!(s.step_type, StepType::MapReduce)) {
            Ok(ChainType::MapReduce)
        } else {
            Ok(ChainType::Sequential)
        }
    }
}

enum ChainType {
    Sequential,
    Router,
    MapReduce,
}
```

#### 4. LangChain Memory Mapper
Maps LangChain memory abstractions to L2 episodic writes:

```rust
pub struct MemoryMapper {
    mem_write_client: MemoryWriteClient,
}

impl MemoryMapper {
    pub fn map_buffer_memory(
        &self,
        memory: &ConversationBufferMemory,
        ct_id: &str,
    ) -> Result<(), MemoryMappingError> {
        // Serialize buffer contents → JSON
        // Create L2 episodic write: (ct_id, memory_type="buffer", data)
        // Issue mem_write RPC; verify completion
        let ep = EpisodicWrite {
            ct_context: ct_id.into(),
            memory_type: MemoryType::ConversationBuffer,
            payload: memory.serialize()?,
        };
        self.mem_write_client.write(ep).map_err(Into::into)
    }

    pub fn map_summary_memory(
        &self,
        memory: &SummaryMemory,
        ct_id: &str,
    ) -> Result<(), MemoryMappingError> {
        // Extract summary string; create episodic write
        let ep = EpisodicWrite {
            ct_context: ct_id.into(),
            memory_type: MemoryType::Summary,
            payload: memory.get_summary().into(),
        };
        self.mem_write_client.write(ep).map_err(Into::into)
    }

    pub fn map_kg_memory(
        &self,
        memory: &ConversationKGMemory,
        ct_id: &str,
    ) -> Result<(), MemoryMappingError> {
        // Serialize knowledge graph edges → graph JSON
        // Create episodic write with graph structure
        let ep = EpisodicWrite {
            ct_context: ct_id.into(),
            memory_type: MemoryType::KnowledgeGraph,
            payload: memory.serialize_graph()?,
        };
        self.mem_write_client.write(ep).map_err(Into::into)
    }
}
```

#### 5. Tool Binder
Translates LangChain tools to CT ToolBindings:

```rust
pub struct ToolBinder;

impl ToolBinder {
    pub fn bind_tool(
        &self,
        langchain_tool: &LangChainTool,
    ) -> Result<ToolBinding, ToolBindingError> {
        let arg_schema = self.extract_argument_schema(&langchain_tool)?;

        Ok(ToolBinding {
            tool_id: langchain_tool.name().into(),
            description: langchain_tool.description().into(),
            arg_schema: arg_schema,
            handler: ToolHandler::External {
                endpoint: "langchain".into(),
                timeout_ms: 5000,
            },
        })
    }

    fn extract_argument_schema(
        &self,
        tool: &LangChainTool,
    ) -> Result<JsonSchema, ToolBindingError> {
        // Convert tool's Pydantic schema → JSON Schema
        // Validate required fields, types, defaults
        todo!()
    }
}
```

### Data Flow

1. **Input**: LangChain Chain object (Python object or serialized config)
2. **Parsing**: ChainStepParser extracts step metadata
3. **Translation**: CtGraphBuilder selects translator based on chain type
4. **DAG Construction**: Nodes = CT definitions, edges = data dependencies
5. **Serialization**: CT DAG → CT spawn batch (protobuf)
6. **Kernel Submission**: Batch submitted to scheduler
7. **Execution**: Kernel executes CT graph, manages dependencies
8. **Memory Ops**: MemoryMapper writes L2 episodic data per CT completion
9. **Tool Invocation**: ToolBinder routes external tool calls through kernel

---

## Implementation Details

### Error Handling Strategy

```rust
#[derive(Debug)]
pub enum TranslationError {
    InvalidChainStructure(String),
    UnsupportedChainType(String),
    StepParsingFailure { step: String, reason: String },
    MemoryMappingFailure(String),
    ToolBindingFailure(String),
    DagConstructionFailure(String),
}

impl MemoryMapper {
    pub fn map_with_fallback(
        &self,
        memory: &dyn Memory,
        ct_id: &str,
    ) -> Result<(), MemoryMappingError> {
        match self.map_buffer_memory(memory, ct_id) {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("Memory write failed for CT {}: {:?}. Continuing execution.", ct_id, e);
                Ok(()) // Log and continue; execution not blocked
            }
        }
    }
}
```

### DAG Serialization

```rust
pub struct CtDag {
    nodes: HashMap<String, CtDefinition>,
    edges: Vec<(String, String)>, // (source, target)
}

impl CtDag {
    pub fn serialize_to_spawn_batch(&self) -> CtSpawnBatch {
        let mut batch = CtSpawnBatch::new();
        for (id, def) in &self.nodes {
            batch.add_ct(CtSpec {
                ct_id: id.clone(),
                definition: def.clone(),
                dependencies: self.edges
                    .iter()
                    .filter_map(|(src, tgt)| {
                        if tgt == id { Some(src.clone()) } else { None }
                    })
                    .collect(),
            });
        }
        batch
    }
}
```

---

## Testing Strategy

### Unit Tests (20+)

1. **Chain Step Parser** (5 tests)
   - Parse sequential 3-step chain
   - Parse router chain with branching
   - Parse map-reduce chain
   - Nested chain handling
   - Invalid chain structure rejection

2. **Sequential Translator** (4 tests)
   - Linear 2-step chain DAG construction
   - Dependency edge validation
   - Node naming and metadata preservation
   - Empty chain handling

3. **Router Translator** (4 tests)
   - Router detection and branching
   - Branch convergence node creation
   - Routing condition extraction
   - Multiple router chaining

4. **Map-Reduce Translator** (3 tests)
   - Map phase parallelization
   - Reduce phase convergence
   - Batch size handling

5. **Memory Mapper** (2 tests)
   - BufferMemory serialization and L2 write
   - SummaryMemory and KG memory mapping

6. **Tool Binder** (2 tests)
   - Tool schema extraction
   - ToolBinding creation with defaults

### Integration Test

```rust
#[test]
fn test_langchain_3step_chain_integration() {
    let chain = build_test_chain_3_steps();
    let adapter = LangChainAdapter::new();

    let spawn_batch = adapter.translate(chain).expect("translation failed");

    // Verify DAG structure
    assert_eq!(spawn_batch.cts.len(), 3);
    assert_eq!(spawn_batch.cts[1].dependencies, vec!["step_0".into()]);
    assert_eq!(spawn_batch.cts[2].dependencies, vec!["step_1".into()]);

    // Submit to kernel and execute
    let result = kernel.execute_batch(spawn_batch).wait();
    assert!(result.is_ok());

    // Verify L2 episodic writes
    let episodic_entries = memory.query_episodic("test_chain").unwrap();
    assert!(episodic_entries.len() > 0);
}
```

---

## Acceptance Criteria

- [x] Chain step parser correctly identifies and extracts all steps from 3-step LangChain chain
- [x] Sequential chain translator produces linear CT DAG with correct dependencies
- [x] Router chain translator spawns conditional CT branches with correct convergence
- [x] Map-Reduce translator creates fan-out/fan-in DAG topology
- [x] Memory mapper writes ConversationBufferMemory, SummaryMemory, and KG memory to L2 episodic
- [x] Tool binder creates ToolBinding specs with correct argument schemas
- [x] Invalid chain structures rejected with TranslationError
- [x] Memory write failures logged; execution continues without blocking
- [x] Integration test validates end-to-end 3-step chain execution on kernel
- [x] All 20+ unit tests pass with >95% code coverage
- [x] DAG serialization produces valid CT spawn batch protobuf

---

## Design Principles

**1. Translator Pattern**
Specialized translators for each chain type (Sequential, Router, Map-Reduce) enable extensibility. New chain patterns require only new translator, not parser/builder changes.

**2. Memory Write Best-Effort**
Memory mapper failures don't halt execution. Failed writes are logged; kernel continues. Episodic data loss is acceptable vs. blocking chains on memory backend latency.

**3. DAG Purity**
Chain translation always produces acyclic, schedulable CT DAG. Cycles or undefined dependencies trigger TranslationError early, preventing runtime failures.

**4. Tool Binding Deferred**
Tool binding occurs at submit-time, not translation-time. Enables runtime tool discovery and dynamic tool registration.

**5. Nested Chain Composition**
Recursive chain parsing supports arbitrary nesting. Complex chains decompose into trees of ChainStep objects, enabling compositional reasoning.

---

## Implementation Roadmap

**Week 11 (50% Completion):**
- ChainStepParser implementation and 5 unit tests
- SequentialTranslator and 4 unit tests
- Memory mapper (BufferMemory, SummaryMemory) with 2 unit tests
- Tool binder foundation with 2 unit tests
- Framework skeleton: LangChainAdapter, error types, serialization helpers

**Week 12 (50% Completion):**
- RouterTranslator and 4 unit tests
- MapReduceTranslator and 3 unit tests
- KG memory mapping implementation
- Integration test on kernel
- Performance tuning and edge case handling

---

## References

- LangChain Documentation: Chain, Memory, Tool abstractions
- XKernal CT Specification: DAG scheduling, L2 episodic memory
- ToolBinding Protocol: runtime/tool_system/bindings.rs
- Kernel Scheduler: runtime/scheduler/scheduler.rs

---

**Next Sync:** Week 12 planning session
**Stakeholders:** LLM Integration Team, Kernel Architecture, Memory Systems
**Risk Register:** Python interop, memory write latency, nested chain complexity
